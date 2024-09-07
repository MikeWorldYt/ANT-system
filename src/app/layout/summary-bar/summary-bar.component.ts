import { AfterViewInit, Component, OnInit } from '@angular/core';
import { CommonModule, NgClass } from '@angular/common';
import { AddHyphenPipe } from '../../pipes/add-hyphen.pipe';
import { Subscription, combineLatest } from 'rxjs';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { ArticleService } from '../../services/navArticleObserver.service';
import { Language } from '../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../docs/content/content';
import { TitleService } from '../../services/navTitle.service';
import { PageService } from '../../services/navPage.service';

@Component({
  selector: 'app-summary-bar',
  standalone: true,
  imports: [
    NgClass,
    CommonModule,
    AddHyphenPipe
  ],
  templateUrl: './summary-bar.component.html',
  styleUrl: './summary-bar.component.css'
})
export class SummaryBarComponent implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = 'ES';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  // ████ OnInit ████
  ngOnInit(): void {
    this.subscribeToChanges();
  }

  // ████ Subscriptions Navigation Context
  subscription = new Subscription();
  private subscribeToChanges(): void {
    this.subscription = combineLatest([
      this.languageService.currentLanguage$,
      this.titleService.currentTitle$,
      this.pageService.currentPage$
    ]).subscribe(([language, title, page]) => {
      this.currentLanguage = language as string;
      this.currentTitle = title as string;
      this.currentPage = page as string;
    });
  }

  // ████ AfterViewInit ████
  ngAfterViewInit(): void {
    // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(id => {
      this.currentArticle = id;
    });
  }


  // ▬▬▬ Toogle Summary
  articleID: string[] = [];
  getPageSummary(id: string): void {
    
  }

  // ████ Map Tooogle Summary
  getSummary(): void {
    const pageContent = content[this.currentLanguage].title_02.page_01;
    this.articleID = Object.keys(pageContent).map((key) => pageContent[key].id);
  }

  // ███ Language Controller

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }


}
