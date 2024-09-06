import { AfterViewInit, Component, OnInit } from '@angular/core';
import { CommonModule, NgClass } from '@angular/common';
import { AddHyphenPipe } from '../../pipes/add-hyphen.pipe';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { ArticleService } from '../../services/navArticleObserver.service';
import { Language } from '../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../docs/content/content';
import { TitleService } from '../../services/navTitle.service';

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
    private intersectionService: ArticleService,
    private languageService: LanguageService,
    private titleService: TitleService,
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'ES';
  currentTitle: string = '';
  // currentPage: string = '';
  currentArticle: string = '';

  // ████ OnInit
  ngOnInit(): void {
    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.currentLanguage = language;
        this.getSummary();
      }
    });
    // Subsribe to Title Service
    this.titleService.currentTitle$.subscribe(title => {
      this.currentTitle = title;
    });
  }

  // ████ AfterViewInit
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

  // ████ Map Tooogle Summary ███
  getSummary(): void {
    const pageContent = content[this.currentLanguage].title_02.page_01;
    this.articleID = Object.keys(pageContent).map((key) => pageContent[key].id);
  }

  // ███ Language Controller ███

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }


}
