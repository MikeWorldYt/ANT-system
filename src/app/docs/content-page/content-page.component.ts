import { Component, ElementRef, Input, QueryList, ViewChildren } from '@angular/core';
import { AddHyphenPipe } from '../../pipes/add-hyphen.pipe';
import { Subscription } from 'rxjs';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { ArticleService } from '../../services/navArticleObserver.service';

// ▲ CONTENT ▲
import { content } from '../content/content';

// ▲ FEATURES ▲
import { HeadingComponent } from '../../features/heading/heading.component';
import { LightboxComponent } from '../../features/lightbox/lightbox.component';
import { ParagraphComponent } from '../../features/paragraph/paragraph.component';
import { CalloutComponent } from '../../features/callout/callout.component';
import { AnchorComponent } from '../../features/anchor/anchor.component';
import { UnorderedListComponent } from '../../features/unordered-list/unordered-list.component';
import { ImagesComponent } from '../../features/images-lightbox/images.component';
import { TableComponent } from '../../features/table/table.component';

@Component({
  selector: 'get-content-page',
  standalone: true,
  imports: [
    HeadingComponent,
    LightboxComponent,
    ParagraphComponent,
    CalloutComponent,
    AnchorComponent,
    UnorderedListComponent,
    ImagesComponent,
    TableComponent,
    AddHyphenPipe,
  ],
  templateUrl: './content-page.component.html',
  styleUrl: '../docs.component.css'
})
export class ContentPageComponent {
  @Input() path!: [string, string, string] ;
  articles: any = {};

  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = '';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  // ▲ service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ███ OnInit ███ 
  ngOnInit(): void {
    this.subscribeToLanguageChanges();
    this.articles = content[this.path[0]][this.path[1]][this.path[2]];
  }

  languageSubscription: Subscription = new Subscription();
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((language: string) => {
      this.currentLanguage = language as string;
    });
  }

  getComponentType(key: string): string {
    const prefix = key.charAt(0);
    switch (prefix) {
      case 'h':
        return 'heading';
      case 'p':
        return 'paragraph';
      case 'i':
        return 'images';
      case 'c':
        return 'callout';
      case 't':
        return 'table-it';
      case 'u':
        return 'unordered-list';
      default:
        return '';
    }
  }

  objectKeys(obj: any): string[] {
    return Object.keys(obj);
  }

  isParentArticle(article: string): boolean {
    return (article.match(/_/g) || []).length === 1; // con un solo "_" principales
  }

  isChildArticle(article: string): boolean {
    return (article.match(/_/g) || []).length === 2; // con dos "_" subartículos
  }

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
    // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(_id => {
      this.currentArticle = _id;
      console.log(this.currentArticle);
    })
  }

}